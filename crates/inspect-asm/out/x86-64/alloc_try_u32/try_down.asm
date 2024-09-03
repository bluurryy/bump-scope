inspect_asm::alloc_try_u32::try_down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	push rax
	mov r14, rsi
	mov rbx, rdi
	mov r15, qword ptr [rsi]
	mov r12, qword ptr [r15]
	mov r13, r12
	and r13, -4
	add r13, -8
	cmp r13, qword ptr [r15 + 8]
	jb .LBB0_4
	mov qword ptr [r15], r13
	test r13, r13
	je .LBB0_4
.LBB0_0:
	call rdx
	mov dword ptr [r13], eax
	lea rbp, [r13 + 4]
	mov dword ptr [r13 + 4], edx
	mov rcx, qword ptr [r14]
	mov rdi, qword ptr [rcx]
	test eax, eax
	je .LBB0_1
	mov rsi, r12
	mov r12d, 1
	cmp r13, rdi
	jne .LBB0_2
	mov rdi, r15
	mov r13d, edx
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov edx, r13d
	mov qword ptr [r14], r15
	jmp .LBB0_2
.LBB0_1:
	xor r12d, r12d
	cmp r13, rdi
	jne .LBB0_2
	mov qword ptr [rcx], rbp
.LBB0_2:
	mov dword ptr [rbx], r12d
	mov dword ptr [rbx + 4], edx
	mov qword ptr [rbx + 8], rbp
.LBB0_3:
	mov rax, rbx
	add rsp, 8
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_4:
	mov rdi, r14
	mov r13, r12
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rdx, r12
	mov r12, r13
	mov r13, rax
	test rax, rax
	jne .LBB0_0
	mov dword ptr [rbx], 2
	jmp .LBB0_3
