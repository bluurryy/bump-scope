inspect_asm::alloc_try_u32::try_up_mut:
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
	test r13, r13
	je .LBB0_4
.LBB0_0:
	call rdx
	mov ebp, edx
	mov dword ptr [r13], eax
	lea rcx, [r13 + 4]
	mov dword ptr [r13 + 4], edx
	test eax, eax
	je .LBB0_1
	mov rdi, r15
	mov rsi, r12
	mov r12, rcx
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov rcx, r12
	mov qword ptr [r14], r15
	mov eax, 1
	jmp .LBB0_2
.LBB0_1:
	add r13, 8
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r13
	xor eax, eax
.LBB0_2:
	mov dword ptr [rbx], eax
	mov dword ptr [rbx + 4], ebp
	mov qword ptr [rbx + 8], rcx
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
	mov r13, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::prepare_allocation_in_another_chunk
	mov rdx, r13
	mov r13, rax
	test rax, rax
	jne .LBB0_0
	mov dword ptr [rbx], 2
	jmp .LBB0_3
