inspect_asm::alloc_try_u32::try_up:
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
	mov rax, qword ptr [r15 + 8]
	lea r13, [r12 + 3]
	and r13, -4
	sub rax, r13
	cmp rax, 8
	jb .LBB0_4
	lea rax, [r13 + 8]
	mov qword ptr [r15], rax
	test r13, r13
	je .LBB0_4
.LBB0_0:
	mov rax, qword ptr [r14]
	mov rbp, qword ptr [rax]
	call rdx
	mov dword ptr [r13], eax
	lea r8, [r13 + 4]
	mov dword ptr [r13 + 4], edx
	mov rcx, qword ptr [r14]
	mov rdi, qword ptr [rcx]
	test eax, eax
	je .LBB0_1
	mov rsi, r12
	mov r12d, 1
	cmp rbp, rdi
	jne .LBB0_2
	mov rdi, r15
	mov ebp, edx
	mov r13, r8
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov r8, r13
	mov edx, ebp
	mov qword ptr [r14], r15
	jmp .LBB0_2
.LBB0_1:
	xor r12d, r12d
	cmp rbp, rdi
	jne .LBB0_2
	add r13, 8
	mov qword ptr [rcx], r13
.LBB0_2:
	mov dword ptr [rbx], r12d
	mov dword ptr [rbx + 4], edx
	mov qword ptr [rbx + 8], r8
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
