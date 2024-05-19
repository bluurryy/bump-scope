inspect_asm::alloc_iter_u32::try_mut_down_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB_12
	mov rbx, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB_2
.LBB_15:
	xor eax, eax
	jmp .LBB_18
.LBB_2:
	mov r14, rsi
	shl rbx, 2
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, rbx
	jb .LBB_4
	add rax, 3
	and rax, -4
	je .LBB_4
.LBB_6:
	sub rdx, rax
	shr rdx, 2
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rdi
	xor r12d, r12d
	mov r15, rsp
	xor edx, edx
	jmp .LBB_7
.LBB_10:
	mov dword ptr [rax + 4*rcx], ebp
	lea rdx, [rcx + 1]
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp rbx, r12
	je .LBB_11
.LBB_7:
	mov ebp, dword ptr [r14 + r12]
	mov rcx, rdx
	cmp qword ptr [rsp + 16], rdx
	jne .LBB_10
	mov rdi, r15
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB_15
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 8]
	jmp .LBB_10
.LBB_11:
	mov rax, qword ptr [rsp + 16]
	test rax, rax
	je .LBB_12
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 24]
	lea r8, [rsi + 4*rdx]
	lea rax, [rsi + 4*rax]
	not rcx
	lea rdi, [rax + 4*rcx]
	cmp rdi, r8
	jae .LBB_16
	mov rax, rsi
	jmp .LBB_17
.LBB_12:
	mov eax, 4
	xor edx, edx
	jmp .LBB_18
.LBB_16:
	lea rax, [4*rdx]
	mov r14, rdx
	mov rdx, rax
	mov r15, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, r15
	mov rdx, r14
.LBB_17:
	mov rcx, qword ptr [rbx]
	mov qword ptr [rcx], rax
.LBB_18:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB_4:
	mov esi, 4
	mov r15, rdi
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB_15
	mov rdi, r15
	jmp .LBB_6