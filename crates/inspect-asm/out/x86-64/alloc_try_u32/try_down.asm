inspect_asm::alloc_try_u32::try_down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	push rax
	mov r15, rsi
	mov rbx, rdi
	mov r12, qword ptr [rsi]
	mov rbp, qword ptr [r12]
	mov r14, rbp
	and r14, -4
	add r14, -8
	cmp r14, qword ptr [r12 + 8]
	jb .LBB0_4
	mov qword ptr [r12], r14
	test r14, r14
	je .LBB0_4
.LBB0_0:
	mov rax, qword ptr [r15]
	mov r13, qword ptr [rax]
	call rdx
	mov dword ptr [r14], eax
	mov dword ptr [r14 + 4], edx
	add r14, 4
	mov rcx, qword ptr [r15]
	mov rdi, qword ptr [rcx]
	test eax, eax
	je .LBB0_1
	mov rsi, rbp
	mov ebp, 1
	cmp r13, rdi
	jne .LBB0_2
	mov rdi, r12
	mov r13d, edx
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov edx, r13d
	mov qword ptr [r15], r12
	jmp .LBB0_2
.LBB0_1:
	xor ebp, ebp
	cmp r13, rdi
	jne .LBB0_2
	mov qword ptr [rcx], r14
.LBB0_2:
	mov dword ptr [rbx], ebp
	mov dword ptr [rbx + 4], edx
	mov qword ptr [rbx + 8], r14
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
	mov rdi, r15
	mov r14, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rdx, r14
	mov r14, rax
	test rax, rax
	jne .LBB0_0
	mov dword ptr [rbx], 2
	jmp .LBB0_3
