inspect_asm::alloc_try_u32::up_mut:
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
	cmp rax, 7
	jbe .LBB0_3
.LBB0_0:
	call rdx
	mov dword ptr [r13], eax
	mov dword ptr [r13 + 4], edx
	test eax, eax
	je .LBB0_1
	mov ebp, edx
	mov rdi, r15
	mov rsi, r12
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov qword ptr [r14], r15
	mov dword ptr [rbx + 4], ebp
	mov eax, 1
	jmp .LBB0_2
.LBB0_1:
	lea rax, [r13 + 4]
	add r13, 8
	mov rcx, qword ptr [r14]
	mov qword ptr [rcx], r13
	mov qword ptr [rbx + 8], rax
	xor eax, eax
.LBB0_2:
	mov dword ptr [rbx], eax
	mov rax, rbx
	add rsp, 8
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_3:
	mov rdi, r14
	mov r13, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::prepare_allocation_in_another_chunk
	mov rdx, r13
	mov r13, rax
	jmp .LBB0_0
